import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
PROTOCOL_PATH = ROOT_DIR / "_vida" / "docs" / "agent-definition-protocol.md"
INSTRUCTION_TEMPLATE_PATH = ROOT_DIR / "_vida" / "templates" / "instruction-contract.yaml"
PROMPT_CONFIG_TEMPLATE_PATH = ROOT_DIR / "_vida" / "templates" / "prompt-template-config.yaml"
GLOSSARY_PATH = ROOT_DIR / "_vida" / "docs" / "research" / "2026-03-08-agentic-terminology-glossary.md"


class AgentDefinitionContractTest(unittest.TestCase):
    def test_protocol_exists(self) -> None:
        self.assertTrue(PROTOCOL_PATH.exists(), "agent-definition protocol must exist")

    def test_instruction_contract_template_declares_required_fields(self) -> None:
        template = INSTRUCTION_TEMPLATE_PATH.read_text(encoding="utf-8")
        required_keys = [
            "contract_id:",
            "version:",
            "role_id:",
            "mission:",
            "scope_boundary:",
            "mandatory_reads:",
            "input_contract:",
            "decision_rules:",
            "allowed_actions:",
            "forbidden_actions:",
            "tool_permission_policy:",
            "fallback_ladder:",
            "escalation_rules:",
            "output_contract:",
            "proof_requirements:",
        ]
        for key in required_keys:
            self.assertIn(key, template, f"instruction contract template missing key: {key}")

    def test_prompt_template_config_declares_required_fields(self) -> None:
        template = PROMPT_CONFIG_TEMPLATE_PATH.read_text(encoding="utf-8")
        required_keys = [
            "config_id:",
            "version:",
            "instruction_contract_ref:",
            "rendering_target:",
            "template_format:",
            "system_prompt_template:",
            "parameter_bindings:",
            "runtime_bindings:",
            "tool_exposure:",
            "output_rendering:",
        ]
        for key in required_keys:
            self.assertIn(key, template, f"prompt template config missing key: {key}")

    def test_protocol_keeps_logic_upstream_of_rendering(self) -> None:
        protocol = PROTOCOL_PATH.read_text(encoding="utf-8")
        self.assertIn("`Instruction Contract` is the canonical logic source.", protocol)
        self.assertIn("`Prompt Template Configuration` is the rendering/configuration layer only.", protocol)
        self.assertIn("Undefined behavior is forbidden by default.", protocol)

    def test_glossary_preserves_hierarchy(self) -> None:
        glossary = GLOSSARY_PATH.read_text(encoding="utf-8")
        self.assertIn("`agent definition`", glossary)
        self.assertIn("`instruction contract`", glossary)
        self.assertIn("`prompt template configuration`", glossary)
        self.assertIn("`instruction contract` is the canonical logic layer", glossary)
        self.assertIn("`prompt template configuration` is the rendering/configuration layer", glossary)


if __name__ == "__main__":
    unittest.main()
