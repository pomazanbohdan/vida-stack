import importlib.util
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-pool.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class SubagentPoolTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("subagent_pool_test", SCRIPT_PATH)

    def test_borrow_subagent_uses_routing_and_lease(self) -> None:
        with mock.patch.object(self.module, "active_pool_leases", return_value={"qwen_cli": {"resource_id": "qwen_cli"}}), \
             mock.patch.object(self.module.subagent_system, "runtime_snapshot", return_value={"agent_system": {}}), \
             mock.patch.object(self.module.subagent_system.vida_config, "load_validated_config", return_value={}), \
             mock.patch.object(self.module.subagent_system, "load_strategy_memory", return_value={}), \
             mock.patch.object(
                 self.module.subagent_system,
                 "route_candidate_context",
                 return_value={"candidates": [{"subagent": "kilo_cli", "effective_score": 72}]},
             ), \
             mock.patch.object(
                 self.module.subagent_system,
                 "acquire_lease",
                 return_value={"status": "acquired", "lease": {"resource_id": "kilo_cli"}},
             ):
            payload = self.module.borrow_subagent("analysis", "holder-1", 600)

        self.assertEqual(payload["status"], "acquired")
        self.assertEqual(payload["selected_subagent"], "kilo_cli")
        self.assertEqual(payload["leased_subagents"], ["qwen_cli"])

    def test_release_subagent_uses_pool_lease_type(self) -> None:
        with mock.patch.object(
            self.module.subagent_system,
            "release_lease",
            return_value={"status": "released", "lease": {"resource_id": "kilo_cli"}},
        ) as mocked_release:
            payload = self.module.release_subagent("kilo_cli", "holder-1")

        mocked_release.assert_called_once_with("subagent_pool", "kilo_cli", "holder-1")
        self.assertEqual(payload["status"], "released")


if __name__ == "__main__":
    unittest.main()
