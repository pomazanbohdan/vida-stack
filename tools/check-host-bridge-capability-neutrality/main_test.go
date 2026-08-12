package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func neutralityFixture(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	paths := []string{
		filepath.Join(root, "crates", "fixture.rs"),
		filepath.Join(root, "scripts", "fixture.ps1"),
		filepath.Join(root, ".github", "workflows", "runtime-quality.yml"),
		filepath.Join(root, "docs", "fixture.generated.md"),
		filepath.Join(root, "vida.config.yaml"),
		filepath.Join(root, "docs", "framework", "templates", "vida.config.yaml.template"),
		filepath.Join(root, "crates", "taskflow-host-bridge", "src", "adapter_contract.rs"),
		filepath.Join(root, "docs", "product", "spec", "host-agent-bridge-adapter-contract.md"),
	}
	for _, path := range paths {
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	contents := map[string]string{
		paths[0]: "pub struct Fixture;\n",
		paths[1]: "Write-Output fixture\n",
		paths[2]: "go build -trimpath -o $binary ./tools/check-host-bridge-capability-neutrality\n& $binary --root $pwd --json\n",
		paths[3]: "generated fixture\n",
		paths[4]: "host_tool_bridge:\n  adapter_kind: fixture.adapter\n  spawn: fixture.spawn\n",
		paths[5]: "host_tool_bridge:\n  adapter_kind: fixture.adapter\n  spawn: fixture.spawn\n",
		paths[6]: "adapter contract\n",
		paths[7]: "\"adapter_operations\"\n\"operations\"\n\"dispose_policy\"\n\"adapter_contract_hash\"\n",
	}
	for path, content := range contents {
		if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

func TestSelfTestPassesCompleteInventory(t *testing.T) {
	root := neutralityFixture(t)
	inv := getSurfaceInventory(root)
	if err := runSelfTest(inv); err != nil {
		t.Fatal(err)
	}
}

func TestEvaluatePassesAllowedConfigAndContract(t *testing.T) {
	root := neutralityFixture(t)
	got := evaluate(root, getSurfaceInventory(root))
	if got.Status != "pass" {
		t.Fatalf("unexpected result: %+v", got)
	}
}

func TestEvaluateBlocksLegacyAlias(t *testing.T) {
	root := neutralityFixture(t)
	if err := os.WriteFile(filepath.Join(root, "scripts", "fixture.ps1"), []byte("spawn_tool\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	got := evaluate(root, getSurfaceInventory(root))
	if got.Status != "blocked" || len(got.Violations) == 0 {
		t.Fatalf("expected legacy alias violation: %+v", got)
	}
}

func TestRunJSONPasses(t *testing.T) {
	root := neutralityFixture(t)
	var output bytes.Buffer
	if err := run([]string{"--root", root, "--json"}, &output, root); err != nil {
		t.Fatal(err)
	}
	var decoded result
	if err := json.Unmarshal(output.Bytes(), &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Status != "pass" || decoded.ScannedSurfaceCount < 4 {
		t.Fatalf("unexpected JSON result: %+v", decoded)
	}
}
