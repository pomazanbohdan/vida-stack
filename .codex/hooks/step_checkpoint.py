"""Ask Codex to optimize the next step every ten completed Stop events."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path


INTERVAL = 10
MESSAGE = (
    "Проаналізуй поточні кроки та внутрішній план наступних дій. "
    "Оптимізуй їх для ефективнішого виконання кроків, викликів "
    "інструментів і агентів, а також паралельного опрацювання, "
    "щоб зменшити витрати токенів."
)


def read_event() -> dict:
    try:
        value = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError, TypeError):
        return {}
    return value if isinstance(value, dict) else {}


def state_path(session_id: str) -> Path:
    digest = hashlib.sha256(session_id.encode("utf-8")).hexdigest()[:32]
    root = Path(
        os.environ.get(
            "CODEX_HOOK_STATE_DIR",
            Path(tempfile.gettempdir()) / "codex-project-step-checkpoints",
        )
    )
    root.mkdir(parents=True, exist_ok=True)
    return root / f"{digest}.json"


def next_count(path: Path) -> int:
    try:
        current = json.loads(path.read_text(encoding="utf-8"))
        count = int(current.get("count", 0))
    except (OSError, ValueError, TypeError, json.JSONDecodeError):
        count = 0

    count += 1
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps({"count": count}), encoding="utf-8")
    os.replace(temporary, path)
    return count


def main() -> None:
    event = read_event()

    # Do not recursively continue the same turn after this hook already did so.
    if event.get("stop_hook_active"):
        print("{}")
        return

    session_id = str(event.get("session_id") or "unknown-session")

    try:
        count = next_count(state_path(session_id))
    except OSError:
        print("{}")
        return

    if count % INTERVAL:
        print("{}")
        return

    print(
        json.dumps(
            {
                "decision": "block",
                "reason": MESSAGE,
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
