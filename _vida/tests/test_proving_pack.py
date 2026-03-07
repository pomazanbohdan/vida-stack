import importlib.util
import json
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "proving-pack.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ProvingPackTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("vida_proving_pack_test", SCRIPT_PATH)

    def test_framework_self_pack_contains_fail_closed_checks(self) -> None:
        payload = self.module.PACKS["framework_self"]
        self.assertIn("goal", payload)
        self.assertTrue(any("fail-closed" in payload["goal"] or "execution auth gate" in item.casefold() for item in payload["checks"]))

    def test_pack_payload_is_json_serializable(self) -> None:
        payload = {
            "generated_at": self.module.now_utc(),
            "pack": "navigation_ownership",
            "task_id": "mobile-1ic.3",
            **self.module.PACKS["navigation_ownership"],
        }
        rendered = json.dumps(payload, sort_keys=True)
        self.assertIn("navigation_ownership", rendered)


if __name__ == "__main__":
    unittest.main()
