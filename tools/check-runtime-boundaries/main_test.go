package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func boundaryFixture(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	src := filepath.Join(root, "crates", "vida", "src")
	if err := os.MkdirAll(src, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(src, "release1_operator_output.rs"), []byte("pub struct Release1;\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	return root
}

func TestEvaluatePassesCleanBoundaryFixture(t *testing.T) {
	got, err := evaluate(boundaryFixture(t), "")
	if err != nil {
		t.Fatal(err)
	}
	if got.Status != "pass" || len(got.Checks) != 4 {
		t.Fatalf("unexpected result: %+v", got)
	}
}

func TestEvaluateBlocksLegacyImport(t *testing.T) {
	root := boundaryFixture(t)
	src := filepath.Join(root, "crates", "vida", "src")
	if err := os.WriteFile(filepath.Join(src, "legacy.rs"), []byte("use crate::operator_contracts::Legacy;\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	got, err := evaluate(root, "")
	if err != nil {
		t.Fatal(err)
	}
	if got.Status != "blocked" || got.Checks[2].Status != "blocked" {
		t.Fatalf("unexpected result: %+v", got)
	}
}

func TestRunJSONReturnsBlockedExitSignal(t *testing.T) {
	root := boundaryFixture(t)
	if err := os.WriteFile(filepath.Join(root, "crates", "vida", "src", "legacy.rs"), []byte("pub(crate) use runtime_dispatch_state::*;\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	if err := run([]string{"--root", root, "--json"}, &output, root); err != errBlocked {
		t.Fatalf("run() error = %v, want errBlocked", err)
	}
	var decoded result
	if err := json.Unmarshal(output.Bytes(), &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Status != "blocked" {
		t.Fatalf("unexpected JSON result: %+v", decoded)
	}
}
