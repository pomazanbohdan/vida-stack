import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "capability-registry.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class CapabilityRegistryTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("capability_registry_test", SCRIPT_PATH)

    def test_implementation_requires_bounded_write(self) -> None:
        registry = {"subagents": {"writer_cli": {"write_scope": "scoped_only", "capability_band": ["bounded_write_safe"]}}}
        payload = self.module.compatibility_for("implementation", "writer_cli", registry)
        self.assertTrue(payload["compatible"])

    def test_problem_party_rejects_bounded_write_lane(self) -> None:
        registry = {"subagents": {"writer_cli": {"write_scope": "scoped_only", "capability_band": ["bounded_write_safe", "review_safe"]}}}
        payload = self.module.compatibility_for("problem_party", "writer_cli", registry)
        self.assertFalse(payload["compatible"])

    def test_build_registry_writes_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            self.module.REGISTRY_PATH = Path(tmpdir) / "capability-registry.json"
            with mock.patch.object(
                self.module.vida_config,
                "load_validated_config",
                return_value={"agent_system": {"subagents": {"review_cli": {"subagent_backend_class": "external_cli", "role": "reviewer", "write_scope": "none", "capability_band": "read_only,review_safe", "specialties": "review,spec", "billing_tier": "free", "speed_tier": "fast", "quality_tier": "high", "dispatch": {"command": "review"}}}}},
            ):
                path = self.module.save_json(self.module.REGISTRY_PATH, self.module.build_registry())
            self.assertTrue(path.exists())
